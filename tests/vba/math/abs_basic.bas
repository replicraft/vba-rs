' INV: func.abs
' SOURCE: spec ms-vbal 6.1.2.10.1.1 Abs
' EXPECT: 5
' EXPECT: 5.5
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Abs(-5))
    Debug.Print CStr(Abs(5.5))
    Debug.Print CStr(Abs(0))
End Sub
