' INV: op.concat, lit.empty
' SOURCE: spec ms-vbal 5.6.9.4 concatenation operator
' EXPECT: x
Public Sub Main()
    Debug.Print CStr(Empty & "x")
End Sub
