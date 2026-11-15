export interface LockState {
  /** false on Linux and on Windows without Windows Hello configured. */
  supported: boolean;
  locked: boolean;
}
