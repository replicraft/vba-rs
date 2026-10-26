' INV: decl.array-bounds
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 9
Public Sub Main()
    Dim a(2) As Long
    a(3) = 1
End Sub
