# Deploy

The site is a static bundle (`web/dist/`) served by **Cloudflare Pages**. Cost: $0
(Pages free tier). GitHub Actions builds the full stack and uploads the artifact —
Cloudflare only serves it. See the header comment in
`.github/workflows/deploy.yml` for why we build in Actions rather than in the
Cloudflare build image.

## How it runs

| Trigger | Result |
|---|---|
| Push to `main` | Production deploy |
| Any pull request (same-repo branch) | Preview deploy; preview URL posted/updated as a PR comment |
| PR from a fork / Dependabot | Deploy job skipped (no access to secrets) |

`web/public/_headers` (MIME + security headers incl. CSP) and
`web/public/_redirects` (SPA fallback so `/learn` loads on refresh) are copied
into `web/dist/` by Vite and applied by Cloudflare Pages automatically.

## One-time maintainer setup

1. **Create the Cloudflare Pages project** (once):
   - Cloudflare dashboard -> *Workers & Pages* -> *Create* -> *Pages* ->
     *Create using direct upload* (Wrangler). Do **not** connect it to the Git
     repo — the GitHub Action pushes builds via `wrangler pages deploy`.
   - Project name: **`strata-studio`** (placeholder — the product name is not
     finalized). If you pick another name it must match `CF_PAGES_PROJECT` in
     `.github/workflows/deploy.yml` (single `env:` value near the top, marked
     `>>> CHANGE ME <<<`).
   - After the first deploy, in *Settings -> Builds & deployments* confirm the
     **Production branch** is `main`. Deploys wrangler makes on any other branch
     become previews automatically.

2. **Create a Cloudflare API token**:
   - *My Profile -> API Tokens -> Create Token -> Custom token*.
   - Permission: **Account -> Cloudflare Pages -> Edit**. Scope it to the one
     account. No zone permissions needed (no custom domain in scope here).
   - Copy the token value.

3. **Find the Cloudflare Account ID**:
   - *Workers & Pages* overview -> right sidebar -> *Account ID* (32 hex chars).

4. **Add two GitHub repository secrets**
   (*repo -> Settings -> Secrets and variables -> Actions -> New repository secret*):
   - `CLOUDFLARE_API_TOKEN` = the token from step 2
   - `CLOUDFLARE_ACCOUNT_ID` = the ID from step 3

5. **First run**: merge this branch, or trigger *Actions -> Deploy ->
   Run workflow* (`workflow_dispatch`). Verify the production URL
   (`https://strata-studio.pages.dev` or your project name) loads the canvas and
   `/<url>/learn` loads on a hard refresh.

## Out of scope here

- Custom domain (add later in Pages *Settings -> Custom domains*; then extend the
  CSP / headers if third-party origins are introduced).
- Rollback: use the Pages dashboard *Deployments* list ("Rollback to this
  deployment").

## Local verification

```sh
cd web
bun install
bun run build          # runs build:wasm (needs wasm-pack + Rust) then vite build
ls dist                # index.html, _headers, _redirects, favicon.svg, assets/
```

`dist/assets/` must contain the hashed `strata_wasm_bg-*.wasm`, the CSS bundle,
and the JS bundles.
