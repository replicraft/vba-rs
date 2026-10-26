' INV: expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 numeric to String Let-coercion
' EXPECT: 12.5!
Public Sub Main()
    Dim s As String
    s = 12.5
    Debug.Print s & "!"
End Sub
