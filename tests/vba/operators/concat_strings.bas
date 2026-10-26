' INV: op.concat
' SOURCE: derived string concatenation
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
