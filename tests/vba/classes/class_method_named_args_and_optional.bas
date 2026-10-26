' INV: expr.named-arg, decl.optional
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  123  129  425 
'#MODULE Calc class
Public Function Mix(ByVal a As Long, Optional ByVal b As Long = 2, Optional ByVal c As Long = 3) As Long
    Mix = a * 100 + b * 10 + c
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim k As New Calc
    Debug.Print k.Mix(1); k.Mix(1, c:=9); k.Mix(c:=5, a:=4)
End Sub
