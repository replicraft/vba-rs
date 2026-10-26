' INV: op.concat
' SOURCE: spec ms-vbal 5.6.9.4 concatenation operator
' EXPECT: ab
' EXPECT: 
' EXPECT: x1
' EXPECT: 12
Public Sub Main()
    Debug.Print CStr("a" & "b")
    Debug.Print CStr("" & "")
    Debug.Print CStr("x" & 1)
    Debug.Print CStr(1 & 2)
End Sub
