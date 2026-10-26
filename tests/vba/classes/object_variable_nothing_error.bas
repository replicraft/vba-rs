' INV: lit.nothing, stmt.set
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' ERROR: 91
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As Box
    b.V = 1
End Sub
