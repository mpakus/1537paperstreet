import type { GraphQuery, GraphSnapshot } from './generated/core'
import { errorMessage, graphGet } from './ipc'

/** Prevents a late graph result from crossing project/query or pane boundaries. */
export class GraphRequest {
  private revision = 0

  invalidate(): void {
    this.revision += 1
  }

  async load(
    projectId: string,
    query: GraphQuery,
    accept: (snapshot: GraphSnapshot) => void,
    fail: (message: string) => void,
  ): Promise<void> {
    const revision = ++this.revision
    try {
      const snapshot = await graphGet(projectId, query)
      if (revision === this.revision) accept(snapshot)
    } catch (cause) {
      if (revision === this.revision) fail(errorMessage(cause))
    }
  }
}
