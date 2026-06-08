import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as child_process from "child_process";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import { loadWoldBindings, createCompletionProvider, createHoverProvider } from "./completions";

let client: LanguageClient | null = null;
let watchProcess: child_process.ChildProcess | null = null;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;
let activeContext: vscode.ExtensionContext | null = null;
let nativeDisposables: vscode.Disposable[] = [];
let nativeProvidersActive = false;

function disposeNativeProviders() {
  for (const d of nativeDisposables) { d.dispose(); }
  nativeDisposables = [];
  nativeProvidersActive = false;
}

function registerNativeProviders(context: vscode.ExtensionContext) {
  if (nativeProvidersActive) return;
  try {
    const wold = loadWoldBindings(context.extensionPath);
    const provider = createCompletionProvider(wold);
    nativeDisposables.push(
      vscode.languages.registerCompletionItemProvider("wolfram", provider)
    );
    const hover = createHoverProvider(wold);
    nativeDisposables.push(
      vscode.languages.registerHoverProvider("wolfram", hover)
    );
    nativeProvidersActive = true;
    outputChannel.appendLine("Native completion + hover providers registered (fallback)");
  } catch (e: any) {
    outputChannel.appendLine(`Native provider setup failed: ${e?.message}`);
  }
}

enum LspStatus { NO_SERVER, STARTING, READY, ERROR }

let lspStatus = LspStatus.NO_SERVER;

function getCompilerPath(): string {
  let p = vscode.workspace.getConfiguration("wolfram").get<string>("compilerPath", "");
  return p.replace(/^["']|["']$/g, "").trim();
}

function getNodePath(): string {
  return process.execPath; // VS Code's built-in Node.js
}

function getLspServerPath(context: vscode.ExtensionContext): string {
  return path.join(context.extensionPath, "out", "lsp", "server.js");
}

function setStatus(s: LspStatus, detail?: string) {
  lspStatus = s;
  switch (s) {
    case LspStatus.NO_SERVER:
      statusBarItem.text = "$(sync~spin) Wolfram: Starting...";
      statusBarItem.tooltip = "Starting TypeScript language server...";
      statusBarItem.command = "wolfram.showOutput";
      break;
    case LspStatus.STARTING:
      statusBarItem.text = "$(sync~spin) Wolfram: Loading...";
      statusBarItem.tooltip = "Loading API bindings and parsing workspace...";
      statusBarItem.command = "wolfram.showOutput";
      break;
    case LspStatus.READY:
      statusBarItem.text = "$(check) Wolfram: Ready";
      statusBarItem.tooltip = "TypeScript language server running. Click for output.";
      statusBarItem.command = "wolfram.showOutput";
      break;
    case LspStatus.ERROR:
      statusBarItem.text = "$(error) Wolfram: Error";
      statusBarItem.tooltip = `LSP error. ${detail || ""} Click to see output.`;
      statusBarItem.command = "wolfram.showOutput";
      break;
  }
}

export function activate(context: vscode.ExtensionContext) {
  console.log("WOLFRAM ACTIVATE START");
  activeContext = context;
  outputChannel = vscode.window.createOutputChannel("Wolfram");
  outputChannel.appendLine("=== Wolfram extension v0.2.0 (TypeScript LSP) ===");
  outputChannel.show();

  statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  context.subscriptions.push(statusBarItem);
  statusBarItem.show();
  setStatus(LspStatus.NO_SERVER);

  console.log("WOLFRAM registering commands");
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.setCompilerPath", () => { vscode.commands.executeCommand("workbench.action.openSettings", "wolfram.compilerPath"); })); } catch(e:any) { console.error(e); outputChannel.appendLine(`setCompilerPath failed: ${e?.message}`); }
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.showOutput", () => { outputChannel.show(); })); } catch(e:any) { console.error(e); outputChannel.appendLine(`showOutput failed: ${e?.message}`); }
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.newProject", () => newProject())); } catch(e:any) { console.error(e); outputChannel.appendLine(`newProject failed: ${e?.message}`); }
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.startWatch", () => startWatch())); } catch(e:any) { console.error(e); outputChannel.appendLine(`startWatch failed: ${e?.message}`); }
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.stopWatch", () => stopWatch())); } catch(e:any) { console.error(e); outputChannel.appendLine(`stopWatch failed: ${e?.message}`); }
  try { context.subscriptions.push(vscode.commands.registerCommand("wolfram.compileFile", () => compileFile())); } catch(e:any) { console.error(e); outputChannel.appendLine(`compileFile failed: ${e?.message}`); }
  console.log("WOLFRAM commands registered");
  outputChannel.appendLine(`Commands registered: ${context.subscriptions.length}`);

  // Start the TypeScript language server
  startLsp();

  // Always register native completion + hover as fallback/supplement
  registerNativeProviders(context);

  // Auto-watch
  const watchOnOpen = vscode.workspace.getConfiguration("wolfram").get<boolean>("watchOnOpen", true);
  if (watchOnOpen && vscode.workspace.workspaceFolders?.length) {
    const ws = vscode.workspace.workspaceFolders[0].uri.fsPath;
    if (checkForWolframFiles(ws)) {
      startWatch();
    }
  }

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (e.affectsConfiguration("wolfram.compilerPath")) {
        outputChannel.appendLine("compilerPath changed, restarting watch if needed");
      }
    })
  );

  // Ctrl+S → single-file transpile (instant feedback without full project rebuild)
  context.subscriptions.push(
    vscode.workspace.onDidSaveTextDocument((doc) => {
      if (doc.languageId !== "wolfram") return;
      const cp = getCompilerPath();
      if (!cp || !fs.existsSync(cp)) return;
      const ws = vscode.workspace.workspaceFolders?.[0];
      if (!ws) return;
      const root = ws.uri.fsPath;
      const relPath = path.relative(root, doc.uri.fsPath);
      if (!relPath || relPath.startsWith("..")) return;
      const outDir = path.join(root, "out");
      try { fs.mkdirSync(outDir, { recursive: true }); } catch {}
      const outFile = path.join(outDir, relPath.replace(/\.\w+\.wrm$|\.wrm$/, ".luau"));
      const parentDir = path.dirname(outFile);
      try { fs.mkdirSync(parentDir, { recursive: true }); } catch {}
      child_process.execFile(cp, [doc.uri.fsPath], { cwd: root }, (err, stdout, stderr) => {
        if (stdout) outputChannel.append(stdout);
        if (stderr) outputChannel.append(stderr);
      });
    })
  );

  outputChannel.appendLine("=== Wolfram extension activated ===");
}

function startLsp() {
  if (client) {
    client.stop();
    client = null;
  }
  if (!activeContext) return;

  const serverPath = getLspServerPath(activeContext);
  if (!fs.existsSync(serverPath)) {
    outputChannel.appendLine(`LSP server not found at "${serverPath}". Run 'npm run compile' first.`);
    outputChannel.appendLine("Falling back to native completion/hover providers.");
    setStatus(LspStatus.ERROR, "server.js not built");
    registerNativeProviders(activeContext);
    return;
  }

  setStatus(LspStatus.STARTING);
  outputChannel.appendLine(`Starting TypeScript LSP: node ${serverPath}`);

  try {
    const node = getNodePath();
    const serverOptions: ServerOptions = {
      run: { command: node, args: [serverPath, "--stdio"] },
      debug: { command: node, args: [serverPath, "--stdio", "--debug"] },
    };
    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: "file", language: "wolfram" }],
      synchronize: { fileEvents: vscode.workspace.createFileSystemWatcher("**/*.wrm") },
      outputChannel,
    };

    client = new LanguageClient("wolfram", "Wolfram", serverOptions, clientOptions);
    activeContext.subscriptions.push(client);

    client.start().then(
      () => {
        outputChannel.appendLine("TypeScript LSP client connected");
        setStatus(LspStatus.READY);
      },
      (err) => {
        outputChannel.appendLine(`LSP start failed: ${err}`);
        setStatus(LspStatus.ERROR, String(err));
        registerNativeProviders(activeContext!);
      }
    );
  } catch (err: any) {
    outputChannel.appendLine(`LSP create failed: ${err?.message || err}`);
    setStatus(LspStatus.ERROR, err?.message || String(err));
    registerNativeProviders(activeContext!);
  }
}

function checkForWolframFiles(dir: string): boolean {
  try {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const p = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "node_modules" && entry.name !== ".git" && entry.name !== "target" && entry.name !== "out") {
          if (checkForWolframFiles(p)) return true;
        }
      } else if (entry.name.endsWith(".wrm")) {
        return true;
      }
    }
  } catch {}
  return false;
}

function startWatch(): void {
  if (watchProcess) return;
  const ws = vscode.workspace.workspaceFolders;
  if (!ws?.length) { vscode.window.showWarningMessage("No workspace open."); return; }

  const cp = getCompilerPath();
  if (!cp || !fs.existsSync(cp)) {
    outputChannel.appendLine("Watch not started: compiler not configured. Set wolfram.compilerPath in Settings for file watching.");
    return;
  }

  const root = ws[0].uri.fsPath;
  outputChannel.appendLine(`Watch: ${cp} --watch "${root}"`);

  watchProcess = child_process.spawn(cp, ["--watch", root]);
  watchProcess.stdout?.on("data", (d: Buffer) => outputChannel.append(d.toString()));
  watchProcess.stderr?.on("data", (d: Buffer) => outputChannel.append(d.toString()));
  watchProcess.on("close", (code) => {
    outputChannel.appendLine(`Watch exited (${code})`);
    watchProcess = null;
  });
  watchProcess.on("error", (err) => {
    outputChannel.appendLine(`Watch error: ${err.message}`);
    vscode.window.showErrorMessage(`Watch failed: ${err.message}`);
    watchProcess = null;
  });
  outputChannel.appendLine("Watch started");
}

function stopWatch(): void {
  if (watchProcess) { watchProcess.kill(); watchProcess = null; outputChannel.appendLine("Watch stopped"); }
}

async function compileFile(): Promise<void> {
  const cp = getCompilerPath();
  if (!cp || !fs.existsSync(cp)) {
    vscode.window.showErrorMessage("Wolfram compiler not found. Set wolfram.compilerPath in Settings.");
    return;
  }

  const ws = vscode.workspace.workspaceFolders?.[0];
  if (!ws) {
    vscode.window.showWarningMessage("No workspace open.");
    return;
  }

  const root = ws.uri.fsPath;
  outputChannel.appendLine(`Compile project: ${cp} "${root}"`);

  child_process.execFile(cp, [root], { cwd: root }, (err, stdout, stderr) => {
    if (stdout) outputChannel.append(stdout);
    if (stderr) outputChannel.append(stderr);
    if (err) {
      vscode.window.showErrorMessage(`Compile failed: ${stderr || err.message}`);
    } else {
      vscode.window.showInformationMessage("Project compiled successfully.");
    }
  });
}

async function newProject(): Promise<void> {
  const folder = await vscode.window.showOpenDialog({
    canSelectFolders: true, canSelectFiles: false,
    openLabel: "Choose folder for new Wolfram project",
  });
  if (!folder?.length) return;

  const name = await vscode.window.showInputBox({
    prompt: "Project name",
    placeHolder: "my-wolfram-game",
    value: path.basename(folder[0].fsPath),
  });
  if (!name) return;

  const src = activeContext!.asAbsolutePath(path.join("templates", "new-project"));
  try {
    copyDir(src, folder[0].fsPath, name);
    vscode.window.showInformationMessage(`Created Wolfram project "${name}"`);
    const main = path.join(folder[0].fsPath, "src", "client", "main.client.wrm");
    if (fs.existsSync(main)) {
      const doc = await vscode.workspace.openTextDocument(main);
      await vscode.window.showTextDocument(doc);
    }
  } catch (err: any) {
    vscode.window.showErrorMessage(`Failed: ${err.message}`);
  }
}

function copyDir(src: string, dst: string, projectName: string) {
  for (const e of fs.readdirSync(src, { withFileTypes: true })) {
    const sp = path.join(src, e.name);
    const dp = path.join(dst, e.name);
    if (e.isDirectory()) {
      fs.mkdirSync(dp, { recursive: true });
      copyDir(sp, dp, projectName);
    } else {
      let content = fs.readFileSync(sp, "utf-8").replace(/\$project_name/g, projectName);
      fs.mkdirSync(path.dirname(dp), { recursive: true });
      fs.writeFileSync(dp, content, "utf-8");
    }
  }
}

export function deactivate(): void {
  stopWatch();
  if (client) { client.stop(); client = null; }
}
