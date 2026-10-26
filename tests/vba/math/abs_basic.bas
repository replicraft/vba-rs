' INV: func.abs
' SOURCE: derived absolute value
' EXPECT: 5
' EXPECT: 5.5
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Abs(-5))
    Debug.Print CStr(Abs(5.5))
    Debug.Print CStr(Abs(0))
End Sub
