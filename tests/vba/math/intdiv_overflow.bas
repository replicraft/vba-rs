' INV: op.intdiv
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 6
Public Sub Main()
    Debug.Print -2147483648# \ -1
End Sub
