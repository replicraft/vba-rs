' INV: decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 9
Public Sub Main()
    Dim a(1 To 3) As Long
    Debug.Print a(0)
End Sub
