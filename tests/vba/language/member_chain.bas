' INV: expr.member
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  4 
'#MODULE Inner class
Public V As Long
'#MODULE Outer class
Public I As New Inner
'#MODULE Module1 standard
Public Sub Main()
    Dim o As New Outer
    o.I.V = 4
    Debug.Print o.I.V
End Sub
