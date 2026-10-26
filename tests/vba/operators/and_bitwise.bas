' INV: op.and
' SOURCE: spec ms-vbal 5.6.9.8.2 And operator
' EXPECT: 8
' EXPECT: 15
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(12 And 10)
    Debug.Print CStr(255 And 15)
    Debug.Print CStr(5 And 0)
End Sub
