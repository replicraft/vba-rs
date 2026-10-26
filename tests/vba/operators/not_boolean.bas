' INV: op.not
' SOURCE: spec ms-vbal 5.6.9.8.1 Not operator
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Not True)
    Debug.Print CStr(Not False)
End Sub
