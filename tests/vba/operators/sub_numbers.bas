' INV: op.sub
' SOURCE: derived subtraction
' EXPECT: 7
' EXPECT: -7
' EXPECT: 0.25
Public Sub Main()
    Debug.Print CStr(10 - 3)
    Debug.Print CStr(3 - 10)
    Debug.Print CStr(0.5 - 0.25)
End Sub
