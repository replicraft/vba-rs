' INV: expr.named-arg, decl.optional
' SOURCE: spec ms-vbal 5.3.1.11 named and unmapped optional arguments, 5.3.1.5 default values
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
