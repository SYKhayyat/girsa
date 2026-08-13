; What uninstalling has to undo, beyond deleting the files it wrote.
;
; Girsa registers `girsa:` as a URL protocol so a citation in a Word document is
; a link that lands on the daf (spec.md §10.6). That registration is a change to
; the machine, and an uninstaller that removes the executable and leaves it
; behind has left a protocol declared with nothing to run — Windows then answers
; a citation with *look for an app in the Store* rather than admitting nothing
; handles it.
;
; Measured after a plain `uninstall.exe /S`: the `shell\open\command` was gone
; and `HKCU\SOFTWARE\Classes\girsa` was still there, declaring
; `URL:org.girsa.app protocol`. This takes the whole key.
;
; **`girsa-endpoint.json` and nothing else in that directory.** The install
; directory and the loopback rendezvous directory are the same folder —
; `%LOCALAPPDATA%\girsa` is where `girsa-post` puts every endpoint file — so
; `ksav-endpoint.json` and `ksav-inbox.jsonl` sit beside ours and belong to the
; other application. Deleting the directory would take Ksav's pairing with it.
; Ours goes because a file naming a dead port is how a sibling concludes we
; crashed; the rest stays because it was never ours to remove.

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegKey HKCU "Software\Classes\girsa"
  Delete "$INSTDIR\girsa-endpoint.json"
!macroend
