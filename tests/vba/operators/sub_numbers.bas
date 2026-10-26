' INV: op.sub
' SOURCE: spec ms-vbal 5.6.9.3.3 binary minus operator
' EXPECT: 7
' EXPECT: -7
' EXPECT: 0.25
Public Sub Main()
    Debug.Print CStr(10 - 3)
    Debug.Print CStr(3 - 10)
    Debug.Print CStr(0.5 - 0.25)
End Sub
