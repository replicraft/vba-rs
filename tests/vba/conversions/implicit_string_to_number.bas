' INV: expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 String to numeric Let-coercion
' EXPECT:  43 
Public Sub Main()
    Dim n As Long
    n = "42"
    Debug.Print n + 1
End Sub
