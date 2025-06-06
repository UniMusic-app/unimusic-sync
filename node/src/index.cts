// This module is the CJS entry point for the library.

import * as addon from "./load.cjs";

declare module "./load.cjs" {
  type NamespaceId = string;
  type AuthorId = string;
  type NodeId = string;
  type Hash = string;
  type DocTicket = string;

  interface FileInfo {
    key: string;
    author: string;
    timestamp: number;
    contentHash: string;
    contentLen: number;
  }

  function initialize(path: string): Promise<void>;
  function shutdown(): Promise<void>;
  function createNamespace(): Promise<string>;
  function deleteNamespace(namespace: NamespaceId): Promise<void>;
  function getAuthor(): Promise<AuthorId>;
  function getNodeId(): Promise<NodeId>;
  function getFiles(namespace: NamespaceId): Promise<FileInfo[]>;
  function writeFile(
    namespace: NamespaceId,
    syncPath: string,
    sourcePath: string
  ): Promise<Hash>;
  function deleteFile(namespace: NamespaceId, syncPath: string): Promise<void>;
  function readFile(
    namespace: NamespaceId,
    syncPath: string
  ): Promise<Uint8Array>;
  function readFileHash(hash: string): Promise<Uint8Array>;
  function exportFile(
    namespace: NamespaceId,
    syncPath: string,
    destinationPath: string
  ): Promise<void>;
  function exportFileHash(hash: string, destinationPath: string): Promise<void>;
  function share(namespace: NamespaceId): Promise<DocTicket>;
  function importTicket(ticket: DocTicket): Promise<NamespaceId>;
  function sync(namespace: NamespaceId): Promise<void>;
  function reconnect(): Promise<void>;
}

export { addon };
