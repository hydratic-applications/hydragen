## Context

This checkout and `fresh-gecko` both start at
`c5d6962d4a4a07c4c88e1ecd90a73fe268446ee5`. The existing implementation offers
project-first author names, groups identical trimmed names, and prepares optional
sources while Name is answered. It freezes choices before Author, with a
15-second preparation lifetime and at most 1,500 ms additional waiting at Author.
Names currently lose their original source records after grouping, and
`Metadata.author` is `Option<String>`. Project contacts and GitHub contact fields
are deliberately discarded.

The user wants the familiar npm author shape, then contact suggestions based on
the selected author. During proposal preparation they clarified two points:

- Skipping Author skips both contact steps and omits the entire author.
- Explicit Author text should recognise npm shorthand; already supplied contact
  fields skip their later steps. Non-shorthand text is a literal name.

The proposal uses public GitHub profile email only, following the user's stated
preference after explaining private-account email and publication risks.

This increment resolves the author-metadata portion of design question 2 and the
guided/automation portion of question 9, while retaining question 11's scoped
responsiveness decision. It supports the existing DX, efficiency/familiarity, and
performance values, rather than adding a new guiding value. Broader author
verification, contributors, and identity-provider policy remain outside scope.

## Goals / Non-Goals

**Goals:**

- Accept existing strings and npm-style author objects in all supported formats.
- Generate objects by default, with a required name and separately optional email
  and URL, without a representation question.
- Reduce typing with explicit shorthand and source-associated contact choices.
- Preserve every distinct associated contact, including disagreement between
  sources with exactly equal names, without making identity claims.
- Keep automated init explicit, offline-capable, prompt-free, and discovery-free.
- Preserve source text on read/add, shared validation, cancellation, and write safety.

**Non-Goals:**

- Multiple authors, contributors, verified identities, mailbox ownership checks,
  reachable-website checks, a separate support-email field, or migration commands.
- Private GitHub email enumeration, deriving email from usernames/domains,
  scraping websites, inspecting commit history/remotes, or new providers.
- OS address-book/directory integrations, new authentication, permission requests,
  account switching, persistent identity caches, or an npm/Node runtime dependency.
- Solving the previously accepted upstream selector defect as part of this work.

## Decisions

### Accept both representations without rewriting existing author data

Use schema definitions for `authorName`, `authorEmail`, `authorUrl`,
`authorObject`, and the string/object union. An object requires `name`; its known
fields are `name`, `email`, and `url`. Omitted optional fields stay absent, not
empty or null. All three values are non-blank strings. Email additionally uses
JSON Schema's `email` format, and URL uses `uri` (an absolute URI, not a relative
path). Enable format assertions explicitly in the bundled offline validator;
Draft 2020-12 annotations alone must not accidentally make these checks inert.
There are no DNS, HTTP, mailbox, or ownership checks and no separate Rust grammar
for content acceptance. A URL is not restricted to HTTP(S) in authored metadata;
for example an absolute `https:` or `mailto:` URI can be represented.

Name reuses the existing Unicode non-blank policy, not the layer-name grammar.
Known contact fields reject wrong types, blank values, and invalid formats.
Unknown author-object properties are accepted and retained as uninterpreted data,
consistent with the existing extensible root object. Init generates only the three
known author fields. Nested duplicate keys remain decoder errors.

The string branch remains the existing non-blank string definition. This preserves
all existing names, including punctuation, Unicode, surrounding whitespace, and
multiline strings. Document npm shorthand as the familiar way to put contacts in
a single string, but do not retroactively reject old strings or normalise them on
load. A legacy string is a literal name unless an input interpretation explicitly
recognises the documented shorthand. A name-only string is always valid.

Use a typed string/object author representation in the library, preserving the
original representation and any extra object data when loading. Validate the
entire decoded document before extraction; remove the current assumption that
author must pass `as_str()`. Keep original decoded data and source for add's
insertion-only verification. Neither loading nor add converts string authors to
objects or vice versa. General library serialization can preserve either form;
the init input routes deliberately construct the object form.

**Alternative:** Tighten every string to a new shorthand grammar. Rejected because
it breaks valid existing names and contradicts the user's literal-name fallback.
Using objects only was rejected because manual shorthand is explicitly wanted.

### Interpret shorthand only at explicit input boundaries

Share one small person-input interpreter between custom Author text, `--author`,
and npm/Cargo source extraction where their documented string forms apply. Keep
interpretation separate from schema content validation. Do not parse explicit
object `name` fields, Git names, account names, or selected name-option labels as
shorthand: those sources have already declared a name.

Recognise the entire input, not an arbitrary substring, as one of:

- `Name <email>`
- `Name (URL)`
- `Name <email> (URL)`

Allow separating whitespace and recognise a URL component only when it has an
explicit URI scheme. Balanced contact delimiters must identify non-empty
components and leave no trailing text. Preserve the exact name component and
contact payloads; remove only delimiter syntax and the separating whitespace
immediately before a recognised suffix. Do not trim the leading name or the
contact payloads. Ordinary name-only input is preserved byte-for-byte. Parsed
components are validated against the maintained schema. Recognised shorthand
with a schema-invalid component gets a useful component-specific error, not
silent fallback to a literal name. Unrecognised or ambiguous syntax is a literal
name for explicit entry, as requested. In project discovery, malformed or
ambiguous person strings remain unusable sources rather than becoming misleading
name suggestions. A valid explicit name is always required to construct an author.

Examples: `Alex <alex@example.com>` supplies email and leaves URL unresolved;
`Alex (https://alex.example)` supplies URL and leaves email unresolved; all three
components supply a complete author; `Example team` and `Alex (team)` are literal
names. A captured component is explicit input, not an inferred suggestion, so its
later prompt is skipped. Do not show a synthetic second completed prompt for it;
the Author answer retains the entered shorthand so the captured values stay
visible. Plain custom names and selected options show their accepted name only.

**Alternative:** Always ask contacts, prefilling shorthand values. Rejected because
the user explicitly wants already supplied fields to skip their steps. Parsing
all names everywhere was rejected because punctuation in an explicit name is not
evidence of contact syntax.

### Collect only unresolved fields, using the selected name group's records

The guided sequence is Name → Author → unresolved Author email → unresolved
Author URL → Description → Layer manifest format. Labels retain `(optional)`.

Author omission immediately goes to Description. Non-empty custom input with no
captured email/URL opens two empty text inputs without suggestions or defaults.
This remains true even when its name exactly matches a discovered group: custom
entry intentionally leaves the discovery selection path. Never reacquire that
group by comparing text. If shorthand captured only URL, ask email and then go to
Description; if it captured only email, ask URL; if it captured both, ask neither.

Selecting an Author candidate retains a typed group identity, not only its name.
For each contact field, derive choices solely from the records in that group.
The email decision does not change the URL pool: entering another email or
skipping it must not hide other source-associated URLs. Different emails and URLs
from the same name group can be chosen independently, with source hints, without
claiming that their combination was present in any one source.

For a field with suggestions, use the native single-choice selector: every
distinct value in stable source order, then `Enter another email` or
`Enter another URL`, then `Skip`. Initially select the first suggestion. Even one
suggestion still gets a selector; accepting it is an explicit choice. Custom
entry opens the shared empty optional field input, with no default, placeholder
value, or return to the selector. Empty input or Skip omits only that field.
With no suggestions, open that text input directly. Whitespace-only and other
schema-invalid input receives field-specific correction and clear-and-Enter skip
guidance. Cancellation at either selector or text input cancels the entire init.

Apply the [stable compact source presentation](../simplify-author-choice-labels/design.md)
to Author, email, and URL selectors. Every visible candidate shows its same dimmed
parenthesised source-family summary, focused or not; only radio markers and name
styling change during navigation. Use comma-separated families in first-source
order, without icons, category swaps, truncation, or separate detail rows. Keep
native scrolling, wrapping, basic-terminal text, and colour controls. Actions have
no suffix and align with raw candidate labels.

Derive contact summaries directly from accepted structured records that actually
supplied that exact contact, not all sources of the selected name and never by
parsing English hints. Deduplicate project metadata, Git, GitHub profile, and
GitHub organisation families while retaining Enterprise host distinctions. A
name-only local-account record cannot become contact evidence. Retain full hints
internally, including the actual contact field (for example `Git local user.email`),
record identities, and provenance even when display families collapse.

Show only the accepted contact value or `Skipped` after completion; custom action
labels and sources are not metadata. Keep typed action values and reset selector
theme state between fields and before the format selector. This reconciles the
contact feature with parent `15c8b38`, superseding this change's earlier focused
full-hint/icon presentation only, without changing discovery or metadata semantics.

**Alternative:** Automatically fill contacts when a name is selected. Rejected
because a name can cover different work/personal identities and a website is not
necessarily the contact URL the developer wants to publish.

### Retain contact evidence before grouping equal names

Extend discovery records with optional validated contacts and their provenance.
Retain record identities through grouping, including project array positions,
Git origin/scope/key-family association, and GitHub host/login. Name groups remain
exact equality after discovered-value trimming, in the existing project-first
order and with existing membership eligibility. Do not case-fold names, join
different display names/logins as aliases, or guess entity type.

For each selected group, group email values and URL values independently by exact
equality after discovery trimming. Do not case-fold emails, normalise URLs, strip
trailing slashes, or merge merely equivalent URI spellings. Retain every distinct
source hint for an equal contact. Order by existing source priority and declaration
order; within one GitHub record put declared website before profile URL.

Discovery trims contacts for suggestions, rejects unsafe/control-bearing text,
and uses the corresponding nested schema definition. An invalid or absent contact
discards that contact only, not an independently valid name or the other contact.
Internal schema failures remain fatal, not an empty discovery result. Explicit
accepted values are not normalised. No credentials or membership/source records
are stored in the layer manifest.

**Alternative:** Keep one email/URL on the grouped name. Rejected because it loses
the conflicting but potentially useful values the user specifically wants to see.

### Extend only the sources already used by author discovery

Source research was checked on 2026-10-20 against the primary references below.
This table states the selected increment, not everything those systems can store.

| Existing source | Email evidence | URL evidence | Association boundary |
| --- | --- | --- | --- |
| `package.json` `author` | Object `email` or `<email>` in shorthand | Object `url` or `(URL)` in shorthand | The same author declaration |
| `Cargo.toml` `package.authors` | Trailing `<email>` | None | The same array entry |
| `pyproject.toml` `project.authors` | Entry's `email` | None | The same entry with a usable `name` |
| `GIT_AUTHOR_NAME` | Explicit `GIT_AUTHOR_EMAIL` | None | The same environment identity; no guessing or fallback from unrelated email variables |
| Git `author.name` / `user.name` | Matching `author.email` / `user.email` | None | Same key family, scope, and origin, only when name association is unambiguous |
| Authenticated GitHub profile | Public `viewer.email` | Declared `blog`/`websiteUrl`, then profile `html_url`/`url` | Same authenticated host/login |
| Accessible GitHub organisation | Public `email` | Declared `websiteUrl`, then organisation `url` | Same organisation record/host/login |
| Local account names and username fallback | None in this increment | None | Names can still join equal-name groups with evidence from other sources |

Project files retain exact-path, regular non-symlink, 1 MiB, parse-as-data rules,
without workspace traversal, execution, or new fields such as project homepage,
repository URL, maintainers, or Python email-only authors. A project's homepage
is not necessarily the author's website.

Expand the existing Git query to collect names and emails with NUL framing,
scope, and origin. Keep all name alternatives and existing precedence. For a
given origin/scope/key-family bucket, attach its distinct emails only when it
contains exactly one distinct usable name. Repeated declarations of that same
name are unambiguous; two distinct names are not, so retain both names without
attaching that bucket's emails. Never zip independent name/email lists or attach
the globally effective email to every historical name. Do not pair across origins,
scopes, author/user families, or environment/configuration boundaries. Keep origin
paths internal and out of hints. This deliberately misses some valid split-file
configurations in return for not suggesting someone else's address. Missing Git
email never removes a name, and the shared Git subprocess budget stays 500 ms.

Reuse the authenticated REST profile lookup for name/login and available website
and profile URL. Extend the existing paged GraphQL query with matching viewer
`email` and organisation `email`, `websiteUrl`, and `url`; `User.email` is explicitly
documented as the publicly visible profile email. Confirm viewer login matches the
REST response before attaching anything. Publish usable names independently of
optional contact enrichment. Missing/null/invalid contact fields leave usable names
intact. Page errors retain earlier records; null optional fields do not poison a
whole record. No per-organisation REST detail requests, retries, or extra email-list
requests are added. Partial viewer contact evidence can be published under the
same host/login before freeze, without creating a duplicate name row.

Prefer a declared website before a GitHub profile URL, but retain both as choices.
Use returned profile URLs where valid; if absent, construct
`https://<selected-host>/<login>` only from the known selected host and that exact
record's usable login. Validate host/login as authority/path components before
construction, so arbitrary string concatenation cannot change the URL host.
Never infer a website from the email domain. Treat a scheme-less website as an
unusable URI for this increment, falling back to the profile URL rather than
inventing a scheme or fetching it. GitHub API URLs are not profile URL suggestions.

The current `whoami` API supplies display names/usernames but no portable email
or website. OS-specific account/contact databases could sometimes supply more,
but introduce new permission, privacy, and platform behaviour. Defer them rather
than fabricate `username@hostname` or interpret account comments as contact data.

**Alternatives:** Read Git's effective author identity only (loses alternatives),
read commit history (unrelated attribution), call every organisation detail endpoint
(latency and rate-limit cost), or use project URLs (not necessarily author URLs).
All are rejected for this increment.

Primary references:

- [npm v12 author forms](https://docs.npmjs.com/cli/v12/configuring-npm/package-json#people-fields-author-contributors)
- [Cargo authors](https://doc.rust-lang.org/cargo/reference/manifest.html#the-authors-field)
- [Python authors](https://packaging.python.org/en/latest/specifications/pyproject-toml/#authors-maintainers)
- [Git configuration and identity variables](https://git-scm.com/docs/git-config#Documentation/git-config.txt-username)
- [GitHub authenticated profile](https://docs.github.com/en/rest/users/users#get-the-authenticated-user)
- [GitHub published GraphQL schema](https://docs.github.com/public/fpt/schema.docs.graphql),
  particularly `User.email`, `User.websiteUrl`, `User.url`, and Organisation fields
- [GitHub email-list permissions](https://docs.github.com/en/rest/users/emails#list-email-addresses-for-the-authenticated-user)
- [whoami 2.1.3 public API](https://docs.rs/whoami/2.1.3/whoami/)

### Treat contact publication as an explicit choice

Use only public-profile GitHub email; do not call `/user/emails`, request email
permissions, enumerate private verified addresses, or manufacture a GitHub noreply
address. Public organisation email may be a general/support contact; label it as
GitHub organisation email, not as a verified personal mailbox or guaranteed support
channel. Empty profile fields simply produce no email suggestion.

Local Git and project email may themselves be private even though available on
disk. Present sources, require selection or explicit entry, and include a concise
README safety note that written author contact details may become public when
the layer manifest is shared. Do not add a modal publication confirmation to each
step. Suggestions do not establish identity, authorship, ownership, or consent of
the named person; the developer remains responsible for attribution.

**Alternative:** Use private addresses when current credentials permit it. Deferred:
it could help someone whose preferred contact is hidden on GitHub, but expands
publication risk and permissions-sensitive behaviour without being necessary here.

### Freeze names and contacts together without another lookup stage

Extend the existing two preparation lanes and snapshot, not a new asynchronous
framework. Keep all resource/output bounds, deadlines, cancellation, and owned
child cleanup. Name is available immediately; only a real Author wait shows
`Finding author suggestions…`. At completion/cutoff close publication and freeze
names and contacts together. Group from that snapshot. No new requests, workers,
wait allowances, or spinners start at Email/URL. Late contact results cannot change
an open or future contact selector. Partial data degrades to empty optional input.

The earlier author feature's accepted cliclack long membership-only selector
limitation remains separate. Preserve its existing regression assertions and
explicit exclusions; do not vendor, fork, or patch the dependency or claim that
this increment fixes the upstream issue.

**Alternative:** Fetch contacts after selecting a name. Rejected because it adds
sequential waits and another cancellation lifecycle when the existing bounded
requests can return those fields before selection.

### Keep automation explicit and generation consistent across formats

Add `--author-email` and `--author-url` as input flags alongside existing ones.
Both require `--author`; the layer name may still use its directory default.
`--author` accepts plain name or recognised shorthand and always becomes an
object. Missing contacts stay absent; no flag-only discovery occurs. When shorthand
and a separate flag supply the same field, accept exactly equal values once and
reject differing values with both flags identified, rather than choosing an
implicit override. Validate every explicit field before deduplicating; an empty
flag is invalid, not a skip. Orphaned contact flags are clap usage failures before
operational dispatch. Component format/conflict failures are typed content/input
errors before writing, without prompts. Use existing diagnostic composition and
escaping, with guided feedback in prompt terms and flag errors naming the flag.

Init creates `author: {name, email?, url?}` even for name-only input. Use nested
block mappings for YAML/YML, two-space indented nested objects for JSON/JSONC,
and a TOML root inline table `author = { name = "Alex", email = "..." }`.
An inline table preserves root `name`, `author`, `description` order and avoids
accidentally placing description inside an `[author]` table. Nested generated
fields use `name`, `email`, `url` order; reading accepts any valid order, including
TOML `[author]` and dotted keys. No new `$schema`, dependencies, placeholders,
format metadata, or unrelated fields are generated. Encode and validate the
complete candidate before opening the exact target, with a trailing newline.

**Alternative:** Leave flags string-only or accept JSON in `--author`. Separate
contact flags are easier to discover and quote, retain a complete automation path,
and avoid adding an author-representation selector or JSON argument mini-language.

## Risks / Trade-offs

- Equal names can refer to different people → Preserve the explicitly requested
  grouping, label actual contact sources, and require independent contact choices;
  never claim verification or silently fill them.
- Conservative Git pairing misses split-origin settings → Prefer no suggestion
  over a false association, retain names, and always allow custom contact entry.
- Shorthand can resemble name punctuation → Recognise only a full unambiguous
  suffix with an explicit URI scheme for URL; otherwise preserve literal input.
- JSON Schema formats can vary between validators → Pin tests to the maintained
  offline validator with format assertions enabled; document syntax, not deliverability.
- More optional steps lengthen init → Skip already supplied contacts and the whole
  contact section on author omission; use selectors only when useful choices exist.
- Additional GitHub fields may be absent/restricted → Keep optional fields independent,
  use partial snapshots, and preserve offline/local operation with no new wait.
- New metadata type breaks library callers/output assumptions → Treat this as an
  explicit API/output change, update all callers and fixtures, and test add with
  string/object authors and extra nested data rather than reserialising them.

## Migration Plan

No existing layer manifest is rewritten. First extend the maintained schema and
typed loading/validation, then generation/input interpretation, source records,
and guided contact collection. Update maintained usage and agent guidance when
runtime behaviour changes. Preserve archived files as historical records.

During implementation update the glossary's layer manifest definition, currently
string-only, without introducing a second author term. This proposal extends the
foundations' existing everyday-development scenario with the supplied example:
the same author name from system/Git/GitHub can have different work/personal
contacts; selecting the name should offer associated contacts rather than guess.
The added paragraph makes the familiarity and publication-privacy consequences
concrete, without inventing a ninth value or storing CLI implementation details
in the foundations. Design-question entries link these proposed decisions and
explicitly distinguish them from implemented behaviour.

Rollback must recognise that an old binary will reject newly generated object
authors. Retain schema/reader support if reverting only the guided contact UX;
otherwise developers must manually convert object authors before using older
binaries. Do not promise backward readability by unmodified older versions.

## Open Questions

No blocking product question remains for this proposed increment. Format checks,
Git pairing conservatism, flag conflict rules, and website/profile ordering are
explicit proposed decisions, not claims about existing behaviour. Private GitHub
addresses, OS contact stores, scheme-less website normalisation, and broader
identity matching remain deferred, without placeholder implementations.
