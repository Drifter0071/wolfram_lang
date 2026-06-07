export interface WoldGlobal {
  name: string;
  type: string;
  description: string;
}

export interface WoldFunction {
  name: string;
  params: WoldParam[];
  returns: string;
  description: string;
}

export interface WoldParam {
  name: string;
  type: string;
  optional?: boolean;
}

export interface WoldProperty {
  name: string;
  type: string;
  rw: boolean;
  description: string;
}

export interface WoldMethod {
  name: string;
  params: WoldParam[];
  returns: string;
  description: string;
}

export interface WoldEvent {
  name: string;
  params: WoldParam[];
  description: string;
}

export interface WoldType {
  name: string;
  description: string;
  extends: string | null;
  tags: string[];
  properties: WoldProperty[];
  methods: WoldMethod[];
  events: WoldEvent[];
}

export interface WoldEnum {
  name: string;
  items: string[];
  description: string;
}

export interface WoldService {
  name: string;
  className: string;
  description: string;
}

export interface WoldFile {
  version: number;
  globals: WoldGlobal[];
  functions: WoldFunction[];
  types: WoldType[];
  enums: WoldEnum[];
  services: WoldService[];
}
