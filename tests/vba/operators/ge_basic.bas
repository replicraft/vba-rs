' INV: op.ge
' SOURCE: spec ms-vbal 5.6.9.5.6 greater-or-equal operator
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(2 >= 2)
    Debug.Print CStr(1 >= 2)
End Sub
