' INV: stmt.redim
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: load
Public Sub Main()
    Dim a(3) As Long
    ReDim a(5)
End Sub
