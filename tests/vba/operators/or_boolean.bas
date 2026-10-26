' INV: op.or
' SOURCE: spec ms-vbal 5.6.9.8.3 Or operator
' EXPECT: True
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(True Or False)
    Debug.Print CStr(False Or False)
End Sub
