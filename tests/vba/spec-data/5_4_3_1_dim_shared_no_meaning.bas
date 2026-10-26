' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 Shared keyword has no meaning
' EXPECT: x=5
Public Sub Main()
    Dim Shared x As Integer
    x = 5
    Debug.Print "x=" & x
End Sub
