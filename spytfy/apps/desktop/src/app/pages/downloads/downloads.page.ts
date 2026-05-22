import { Component, ChangeDetectionStrategy, signal, OnInit, OnDestroy } from '@angular/core';
import {
  listBatches, listJobs, pauseBatch, resumeBatch, cancelBatch, retryJob, retryAllFailed, resumeQueued, pickCandidate,
  onJobState, onBatchProgress, onBatchComplete, onDownloadJobProgress, onJobCover,
  type JobStateEvent, type BatchProgressEvent, type DownloadProgressEvent, type JobCoverEvent,
} from '@spytfy/tauri-ipc';
import type { Batch, DownloadJob } from '@spytfy/models';
import type { UnlistenFn } from '@tauri-apps/api/event';
import { ToastService } from '../../layout/toast.component';

@Component({
  selector: 'spytfy-downloads-page',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div class="page animate-in">
      <div class="page-header">
        <h1>Downloads</h1>
        @if (hasStuckJobs()) {
          <button class="resume-pill" (click)="onResumeAll()">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>
            Resume {{ stuckCount() }} queued
          </button>
        }
      </div>

      @if (batches().length === 0) {
        <div class="empty">
          <div class="empty-visual">
            <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" x2="12" y1="15" y2="3"/>
            </svg>
          </div>
          <h2>No downloads yet</h2>
          <p>Paste a Spotify link on Home to get started</p>
        </div>
      }

      @for (batch of batches(); track batch.id) {
        <div class="batch">
          <div class="batch-header">
            <div class="batch-meta">
              <span class="batch-type">{{ batch.sourceType }}</span>
              <h2 class="batch-name">{{ batch.name }}</h2>
              <span class="batch-stats">{{ batchStats(batch.id) }}</span>
            </div>
            <div class="batch-controls">
              @if (hasFailedJobs(batch.id)) {
                <button class="ctrl-btn accent" (click)="onRetryAll(batch.id)" title="Retry all failed">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10"/></svg>
                </button>
              }
              @if (batch.state === 'active') {
                @if (hasQueuedJobs(batch.id)) {
                  <button class="ctrl-btn accent" (click)="onResumeBatch(batch.id)" title="Resume queued">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                  </button>
                }
                <button class="ctrl-btn" (click)="onPause(batch.id)" title="Pause">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>
                </button>
                <button class="ctrl-btn danger" (click)="onCancel(batch.id)" title="Cancel">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M18 6 6 18"/><path d="m6 6 12 12"/></svg>
                </button>
              } @else if (batch.state === 'paused') {
                <button class="ctrl-btn accent" (click)="onResume(batch.id)" title="Resume">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><polygon points="5 3 19 12 5 21 5 3"/></svg>
                </button>
              }
            </div>
          </div>

          <div class="job-list">
            @for (job of getJobs(batch.id); track job.id) {
              <div class="job" [class]="'state-' + job.state">
                @if (job.coverUrl) {
                  <img [src]="job.coverUrl" alt="" class="job-art" />
                } @else {
                  <div class="job-art-empty">
                    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="3"/></svg>
                  </div>
                }
                <div class="job-meta">
                  <span class="job-title">{{ job.title }}</span>
                  <span class="job-artist">{{ job.artist }}</span>
                </div>
                <div class="job-status">
                  @switch (job.state) {
                    @case ('queued') { <span class="pill muted">Queued</span> }
                    @case ('pending') { <span class="pill muted">Queued</span> }
                    @case ('resolving') { <span class="pill active"><span class="eq-bars"><span></span><span></span><span></span></span>Searching</span> }
                    @case ('downloading') {
                      <div class="dl-status">
                        <span class="pill active"><span class="eq-bars"><span></span><span></span><span></span></span>Downloading</span>
                        @if (jobProgress().get(job.id); as pct) {
                          <div class="micro-bar"><div class="micro-fill" [style.width.%]="pct"></div></div>
                        }
                      </div>
                    }
                    @case ('tagging') { <span class="pill active"><span class="eq-bars"><span></span><span></span><span></span></span>Tagging</span> }
                    @case ('verifying') { <span class="pill active"><span class="eq-bars"><span></span><span></span><span></span></span>Verifying</span> }
                    @case ('done') {
                      <span class="pill done">
                        <svg class="check" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M20 6 9 17l-5-5"/></svg>
                        Done
                      </span>
                    }
                    @case ('done_warning') {
                      <span class="pill warn" [title]="job.error || ''">Done</span>
                    }
                    @case ('needs_review') {
                      <span class="pill warn">Review</span>
                      <button class="mini-btn" (click)="toggleReview(job.id)">
                        {{ expandedReview() === job.id ? 'Hide' : 'Pick' }}
                      </button>
                    }
                    @case ('failed') {
                      <span class="pill fail">Failed</span>
                      <button class="mini-btn" (click)="onRetry(job.id)">Retry</button>
                      @if (job.error) {
                        <span class="err-text" [title]="job.error">{{ job.error }}</span>
                      }
                    }
                  }
                </div>
              </div>
              @if (job.state === 'needs_review' && expandedReview() === job.id) {
                <div class="candidates">
                  @for (c of parseCandidates(job.candidatesJson); track c.url) {
                    <button class="candidate" (click)="onPickCandidate(job.id, c.url)">
                      <div class="cand-info">
                        <span class="cand-title">{{ c.title }}</span>
                        <span class="cand-meta">{{ c.uploader }} · {{ formatSecs(c.durationSecs) }} · score {{ c.score }}</span>
                      </div>
                      <span class="cand-pick">Use</span>
                    </button>
                  }
                  <button class="mini-btn" style="margin-top:4px" (click)="onRetry(job.id)">Search again</button>
                </div>
              }
            }
          </div>
        </div>
      }
    </div>
  `,
  styles: `
    .page {
      padding: 32px 40px;
      max-width: 860px;
      min-height: 100%;
    }
    .page.animate-in { animation: slideUp var(--duration-slow) var(--ease-out) both; }

    .page-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 32px;
    }
    h1 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 28px;
      font-weight: 800;
      letter-spacing: -0.04em;
    }
    .resume-pill {
      display: flex;
      align-items: center;
      gap: 8px;
      height: 40px;
      padding: 0 20px;
      background: var(--accent);
      border: none;
      border-radius: var(--radius-pill);
      color: #000;
      font-family: inherit;
      font-size: 13px;
      font-weight: 700;
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .resume-pill:hover { background: var(--accent-hover); transform: scale(1.03); }

    .empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: calc(100vh - 160px);
      gap: 12px;
      text-align: center;
    }
    .empty-visual {
      width: 96px;
      height: 96px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.04);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      margin-bottom: 8px;
    }
    .empty h2 {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 20px;
      font-weight: 700;
      color: var(--text-secondary);
    }
    .empty p { font-size: 14px; color: var(--text-muted); }

    .batch {
      background: var(--surface-700);
      border-radius: var(--radius-lg);
      margin-bottom: 16px;
      overflow: hidden;
      animation: slideUp var(--duration-slow) var(--ease-out) both;
    }
    .batch-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 20px 24px;
    }
    .batch-meta { min-width: 0; }
    .batch-type {
      font-size: 11px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      color: var(--accent);
      display: block;
      margin-bottom: 4px;
    }
    .batch-name {
      font-family: 'Space Grotesk', sans-serif;
      font-size: 17px;
      font-weight: 700;
      letter-spacing: -0.02em;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .batch-stats { font-size: 13px; color: var(--text-muted); margin-top: 2px; display: block; }
    .batch-controls { display: flex; gap: 6px; flex-shrink: 0; }
    .ctrl-btn {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      border: none;
      background: rgba(255, 255, 255, 0.07);
      color: var(--text-secondary);
      cursor: pointer;
      display: flex;
      align-items: center;
      justify-content: center;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .ctrl-btn:hover { background: rgba(255, 255, 255, 0.12); color: var(--text-primary); }
    .ctrl-btn.accent:hover { background: var(--accent); color: #000; }
    .ctrl-btn.danger:hover { background: var(--error); color: #fff; }

    .job-list {
      max-height: 420px;
      overflow-y: auto;
    }
    .job-list::-webkit-scrollbar { width: 6px; }
    .job-list::-webkit-scrollbar-track { background: transparent; }
    .job-list::-webkit-scrollbar-thumb { background: rgba(255,255,255,0.08); border-radius: 3px; }
    .job {
      display: flex;
      align-items: center;
      gap: 12px;
      padding: 10px 24px;
      transition: background var(--duration-fast) var(--ease-out);
    }
    .job:hover { background: rgba(255, 255, 255, 0.03); }
    .job-art {
      width: 44px;
      height: 44px;
      border-radius: var(--radius-sm);
      object-fit: cover;
      flex-shrink: 0;
    }
    .job-art-empty {
      width: 44px;
      height: 44px;
      border-radius: var(--radius-sm);
      background: var(--surface-600);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-muted);
      flex-shrink: 0;
    }
    .job-meta { flex: 1; min-width: 0; }
    .job-title {
      font-size: 14px;
      font-weight: 500;
      display: block;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .job:hover .job-title { color: #fff; }
    .job-artist {
      font-size: 13px;
      color: var(--text-muted);
      display: block;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .job-status {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
      margin-left: 12px;
    }

    .pill {
      font-size: 11px;
      font-weight: 600;
      padding: 4px 10px;
      border-radius: var(--radius-pill);
      display: inline-flex;
      align-items: center;
      gap: 6px;
    }
    .pill.muted { background: rgba(255,255,255,0.06); color: var(--text-muted); }
    .pill.active { background: var(--success-subtle); color: var(--accent); }
    .pill.done { background: var(--success-subtle); color: var(--accent); }
    .pill.warn { background: var(--warning-subtle); color: var(--warning); }
    .pill.fail { background: var(--error-subtle); color: var(--error); }

    .eq-bars {
      display: inline-flex;
      align-items: flex-end;
      gap: 2px;
      height: 10px;
    }
    .eq-bars span {
      display: block;
      width: 2px;
      border-radius: 1px;
      background: var(--accent);
    }
    .eq-bars span:nth-child(1) { animation: eqbar 0.8s ease-in-out infinite 0s; }
    .eq-bars span:nth-child(2) { animation: eqbar 0.8s ease-in-out infinite 0.15s; }
    .eq-bars span:nth-child(3) { animation: eqbar 0.8s ease-in-out infinite 0.3s; }
    @keyframes eqbar {
      0%, 100% { height: 3px; }
      50% { height: 10px; }
    }

    .dl-status { display: flex; align-items: center; gap: 8px; }
    .micro-bar {
      width: 56px;
      height: 3px;
      background: var(--surface-500);
      border-radius: 2px;
      overflow: hidden;
    }
    .micro-fill {
      height: 100%;
      background: var(--accent);
      border-radius: 2px;
      transition: width 300ms ease;
    }
    .mini-btn {
      font-size: 11px;
      padding: 4px 10px;
      border-radius: var(--radius-pill);
      border: none;
      background: rgba(255, 255, 255, 0.07);
      color: var(--text-secondary);
      cursor: pointer;
      font-weight: 600;
      transition: all var(--duration-fast) var(--ease-out);
    }
    .mini-btn:hover { background: var(--accent); color: #000; }
    .err-text {
      font-size: 11px;
      color: var(--error);
      max-width: 180px;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .check {
      stroke-dasharray: 30;
      stroke-dashoffset: 30;
      animation: checkmark-draw 400ms var(--ease-out) forwards 200ms;
    }
    .job.state-done { opacity: 0.6; }
    .job.state-done:hover { opacity: 1; }

    .candidates {
      padding: 8px 24px 16px;
      display: flex;
      flex-direction: column;
      gap: 4px;
      background: rgba(0, 0, 0, 0.2);
    }
    .candidate {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 14px;
      background: var(--surface-700);
      border: 1px solid transparent;
      border-radius: var(--radius-md);
      cursor: pointer;
      transition: all var(--duration-fast) var(--ease-out);
      text-align: left;
      font-family: inherit;
      color: var(--text-primary);
    }
    .candidate:hover {
      border-color: var(--accent);
      background: var(--surface-600);
    }
    .cand-info { min-width: 0; flex: 1; }
    .cand-title {
      font-size: 13px;
      font-weight: 500;
      display: block;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .cand-meta { font-size: 11px; color: var(--text-muted); display: block; margin-top: 2px; }
    .cand-pick {
      font-size: 12px;
      font-weight: 700;
      color: var(--accent);
      flex-shrink: 0;
      margin-left: 12px;
    }

    @media (prefers-reduced-motion: reduce) {
      .page.animate-in, .batch { animation: none; }
      .eq-bars span { animation: none; }
      .eq-bars span:nth-child(1) { height: 4px; }
      .eq-bars span:nth-child(2) { height: 8px; }
      .eq-bars span:nth-child(3) { height: 6px; }
    }
  `,
})
export class DownloadsPage implements OnInit, OnDestroy {
  batches = signal<Batch[]>([]);
  jobs = signal<DownloadJob[]>([]);
  stuckCount = signal(0);
  jobProgress = signal<Map<string, number>>(new Map());
  expandedReview = signal<string | null>(null);

  hasStuckJobs = () => this.stuckCount() > 0;

  constructor(private toast: ToastService) {}

  private unlisteners: UnlistenFn[] = [];

  async ngOnInit() {
    await this.refresh();

    this.unlisteners.push(
      await onJobState((e) => this.handleJobState(e)),
      await onBatchProgress((e) => this.handleBatchProgress(e)),
      await onBatchComplete((e) => { this.refresh(); this.toast.success(`Batch complete — ${e.done} downloaded`); }),
      await onDownloadJobProgress((e) => this.handleDownloadProgress(e)),
      await onJobCover((e) => this.handleJobCover(e)),
    );
  }

  ngOnDestroy() {
    this.unlisteners.forEach((fn) => fn());
  }

  async refresh() {
    try {
      const [b, j] = await Promise.all([listBatches(50), listJobs()]);
      this.batches.set(b);
      this.jobs.set(j);
      const stuck = j.filter(job => ['queued', 'pending', 'resolving', 'downloading'].includes(job.state)).length;
      this.stuckCount.set(stuck);
    } catch {}
  }

  async onResumeAll() {
    await resumeQueued();
    this.stuckCount.set(0);
    await this.refresh();
  }

  getJobs(batchId: string): DownloadJob[] {
    return this.jobs().filter((j) => j.batchId === batchId);
  }

  batchStats(batchId: string): string {
    const jobs = this.getJobs(batchId);
    const done = jobs.filter((j) => j.state === 'done' || j.state === 'done_warning').length;
    const failed = jobs.filter((j) => j.state === 'failed').length;
    const review = jobs.filter((j) => j.state === 'needs_review').length;
    const total = jobs.length;
    let s = `${done}/${total} done`;
    if (failed > 0) s += ` · ${failed} failed`;
    if (review > 0) s += ` · ${review} needs review`;
    return s;
  }

  handleJobState(e: JobStateEvent) {
    this.jobs.update((jobs) =>
      jobs.map((j) =>
        j.id === e.jobId ? { ...j, state: e.state as any, error: e.error } : j
      )
    );
    if (e.state !== 'downloading') {
      this.jobProgress.update((m) => {
        const next = new Map(m);
        next.delete(e.jobId);
        return next;
      });
    }
  }

  handleJobCover(e: JobCoverEvent) {
    this.jobs.update((jobs) =>
      jobs.map((j) =>
        j.id === e.jobId ? { ...j, coverUrl: e.coverUrl } : j
      )
    );
  }

  handleBatchProgress(_e: BatchProgressEvent) {}

  handleDownloadProgress(e: DownloadProgressEvent) {
    this.jobProgress.update((m) => {
      const next = new Map(m);
      next.set(e.jobId, e.percent);
      return next;
    });
  }

  async onPause(batchId: string) {
    await pauseBatch(batchId);
    this.batches.update((bs) => bs.map((b) => b.id === batchId ? { ...b, state: 'paused' } : b));
  }

  async onResume(batchId: string) {
    await resumeBatch(batchId);
    this.batches.update((bs) => bs.map((b) => b.id === batchId ? { ...b, state: 'active' } : b));
  }

  async onCancel(batchId: string) {
    await cancelBatch(batchId);
    this.toast.show('Batch cancelled');
    await this.refresh();
  }

  hasQueuedJobs(batchId: string): boolean {
    return this.getJobs(batchId).some(j => j.state === 'queued' || j.state === 'pending');
  }

  hasFailedJobs(batchId: string): boolean {
    return this.getJobs(batchId).some(j => j.state === 'failed');
  }

  async onResumeBatch(batchId: string) {
    await resumeBatch(batchId);
    await this.refresh();
  }

  async onRetryAll(batchId: string) {
    await retryAllFailed(batchId);
    await this.refresh();
  }

  async onRetry(jobId: string) {
    await retryJob(jobId);
    await this.refresh();
  }

  toggleReview(jobId: string) {
    this.expandedReview.set(this.expandedReview() === jobId ? null : jobId);
  }

  parseCandidates(json?: string): { url: string; title: string; uploader: string; durationSecs: number; score: number }[] {
    if (!json) return [];
    try { return JSON.parse(json); } catch { return []; }
  }

  formatSecs(secs: number): string {
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  async onPickCandidate(jobId: string, ytUrl: string) {
    this.expandedReview.set(null);
    await pickCandidate(jobId, ytUrl);
    await this.refresh();
  }
}
