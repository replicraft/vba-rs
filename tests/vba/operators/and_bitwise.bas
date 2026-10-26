' INV: op.and
' SOURCE: derived bitwise And on integers
' EXPECT: 8
' EXPECT: 15
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(12 And 10)
    Debug.Print CStr(255 And 15)
    Debug.Print CStr(5 And 0)
End Sub
