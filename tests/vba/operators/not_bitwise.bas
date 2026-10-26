' INV: op.not
' SOURCE: spec ms-vbal 5.6.9.8.1 Not operator
' EXPECT: -6
' EXPECT: -1
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Not 5)
    Debug.Print CStr(Not 0)
    Debug.Print CStr(Not -1)
End Sub
