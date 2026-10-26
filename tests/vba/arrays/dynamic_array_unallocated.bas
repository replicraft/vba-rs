' INV: stmt.redim, func.ubound
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 9
Public Sub Main()
    Dim a() As Long
    Debug.Print UBound(a)
End Sub
