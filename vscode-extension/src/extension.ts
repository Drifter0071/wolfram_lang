import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as child_process from "child_process";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable,
} from "vscode-languageclient/node";

let client: LanguageClient;
let watchProcess: child_process.ChildProcess | null = null;
let statusBarItem: vscode.StatusBarItem;
let outputChannel: vscode.OutputChannel;

function getCompilerPath(): string {
  const config = vscode.workspace.getConfiguration("wolfram");
  return config.get<string>("compilerPath", "wolfram");
}

function getOutputDir(): string {
  const config = vscode.workspace.getConfiguration("wolfram");
  return config.get<string>("outputDir", "out");
}

export function activate(context: vscode.ExtensionContext) {
  outputChannel = vscode.window.createOutputChannel("Wolfram");
  outputChannel.appendLine("Wolfram extension activated");

  statusBarItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBarItem.command = "wolfram.toggleWatch";
  statusBarItem.text = "$(circle-outline) Wolfram: Idle";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  const compilerPath = getCompilerPath();
  const serverOptions: ServerOptions = {
    run: {
      command: compilerPath,
      args: ["lsp"],
    } as Executable,
    debug: {
      command: compilerPath,
      args: ["lsp"],
    } as Executable,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "wolfram" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.wrm"),
    },
    outputChannel: outputChannel,
  };

  client = new LanguageClient(
    "wolfram",
    "Wolfram Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
  context.subscriptions.push(client);

  context.subscriptions.push(
    vscode.commands.registerCommand("wolfram.newProject", () =>
      newProject(context)
    )
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("wolfram.startWatch", () => startWatch())
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("wolfram.stopWatch", () => stopWatch())
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("wolfram.compileFile", () => compileFile())
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("wolfram.toggleWatch", () => toggleWatch())
  );

  const watchOnOpen = vscode.workspace
    .getConfiguration("wolfram")
    .get<boolean>("watchOnOpen", true);

  if (
    watchOnOpen &&
    vscode.workspace.workspaceFolders &&
    vscode.workspace.workspaceFolders.length > 0
  ) {
    const workspaceRoot = vscode.workspace.workspaceFolders[0].uri.fsPath;
    const hasWolframFiles = checkForWolframFiles(workspaceRoot);
    if (hasWolframFiles) {
      startWatch();
    }
  }
}

function checkForWolframFiles(dir: string): boolean {
  try {
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (entry.name !== "node_modules" && entry.name !== ".git" && entry.name !== "target") {
          if (checkForWolframFiles(fullPath)) return true;
        }
      } else if (entry.name.endsWith(".wrm")) {
        return true;
      }
    }
  } catch {
    // ignore permission errors
  }
  return false;
}

function startWatch(): void {
  if (watchProcess) return;

  const workspaceFolders = vscode.workspace.workspaceFolders;
  if (!workspaceFolders || workspaceFolders.length === 0) {
    vscode.window.showWarningMessage("No workspace folder open.");
    return;
  }

  const workspaceRoot = workspaceFolders[0].uri.fsPath;
  const compilerPath = getCompilerPath();

  outputChannel.appendLine(`Starting watch server: ${compilerPath} --watch "${workspaceRoot}"`);

  watchProcess = child_process.spawn(compilerPath, [
    "--watch",
    workspaceRoot,
  ]);

  watchProcess.stdout?.on("data", (data: Buffer) => {
    outputChannel.append(data.toString());
    statusBarItem.text = "$(eye) Wolfram: Watching";
  });

  watchProcess.stderr?.on("data", (data: Buffer) => {
    outputChannel.append(data.toString());
  });

  watchProcess.on("close", (code: number | null) => {
    outputChannel.appendLine(`Watch server exited with code ${code}`);
    watchProcess = null;
    statusBarItem.text = "$(circle-outline) Wolfram: Idle";
  });

  watchProcess.on("error", (err: Error) => {
    outputChannel.appendLine(`Watch server error: ${err.message}`);
    vscode.window.showErrorMessage(
      `Failed to start Wolfram watch server: ${err.message}`
    );
    watchProcess = null;
  });

  statusBarItem.text = "$(eye) Wolfram: Watching";
}

function stopWatch(): void {
  if (!watchProcess) return;
  watchProcess.kill();
  watchProcess = null;
  statusBarItem.text = "$(circle-outline) Wolfram: Idle";
  outputChannel.appendLine("Watch server stopped.");
}

function toggleWatch(): void {
  if (watchProcess) {
    stopWatch();
  } else {
    startWatch();
  }
}

async function compileFile(): Promise<void> {
  const editor = vscode.window.activeTextEditor;
  if (!editor || editor.document.languageId !== "wolfram") {
    vscode.window.showWarningMessage("No active Wolfram file to compile.");
    return;
  }

  const compilerPath = getCompilerPath();
  const filePath = editor.document.uri.fsPath;
  const workspaceRoot =
    vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? ".";

  outputChannel.appendLine(`Compiling: ${filePath}`);

  child_process.execFile(
    compilerPath,
    [filePath],
    { cwd: workspaceRoot },
    (error, stdout, stderr) => {
      if (error) {
        outputChannel.append(stderr || error.message);
        vscode.window.showErrorMessage(
          `Compilation failed: ${stderr || error.message}`
        );
        return;
      }
      outputChannel.append(stdout);
      vscode.window.showInformationMessage("File compiled successfully.");
    }
  );
}

async function newProject(context: vscode.ExtensionContext): Promise<void> {
  const folder = await vscode.window.showOpenDialog({
    canSelectFolders: true,
    canSelectFiles: false,
    openLabel: "Choose folder for new Wolfram project",
  });

  if (!folder || folder.length === 0) return;

  const targetDir = folder[0].fsPath;

  const projectName = await vscode.window.showInputBox({
    prompt: "Project name",
    placeHolder: "my-wolfram-game",
    value: path.basename(targetDir),
  });

  if (!projectName) return;

  const templateDir = context.asAbsolutePath(
    path.join("templates", "new-project")
  );

  try {
    copyTemplateFiles(templateDir, targetDir, projectName);
    vscode.window.showInformationMessage(
      `Wolfram project "${projectName}" created at ${targetDir}`
    );

    const mainFile = path.join(targetDir, "src", "client", "main.client.wrm");
    if (fs.existsSync(mainFile)) {
      const doc = await vscode.workspace.openTextDocument(mainFile);
      await vscode.window.showTextDocument(doc);
    }
  } catch (err) {
    vscode.window.showErrorMessage(
      `Failed to create project: ${err instanceof Error ? err.message : String(err)}`
    );
  }
}

function copyTemplateFiles(
  templateDir: string,
  targetDir: string,
  projectName: string
): void {
  const entries = fs.readdirSync(templateDir, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(templateDir, entry.name);
    const dstPath = path.join(targetDir, entry.name);

    if (entry.isDirectory()) {
      if (!fs.existsSync(dstPath)) {
        fs.mkdirSync(dstPath, { recursive: true });
      }
      copyTemplateFiles(srcPath, dstPath, projectName);
    } else {
      let content = fs.readFileSync(srcPath, "utf-8");
      content = content.replace(/\$project_name/g, projectName);
      const dstDir = path.dirname(dstPath);
      if (!fs.existsSync(dstDir)) {
        fs.mkdirSync(dstDir, { recursive: true });
      }
      fs.writeFileSync(dstPath, content, "utf-8");
    }
  }
}

export function deactivate(): void {
  stopWatch();
  if (client) {
    client.stop();
  }
  if (outputChannel) {
    outputChannel.dispose();
  }
}
