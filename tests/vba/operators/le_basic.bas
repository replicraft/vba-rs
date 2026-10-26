' INV: op.le
' SOURCE: spec ms-vbal 5.6.9.5.5 less-or-equal operator
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(2 <= 2)
    Debug.Print CStr(3 <= 2)
End Sub
