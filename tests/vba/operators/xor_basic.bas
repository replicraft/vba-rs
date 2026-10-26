' INV: op.xor
' SOURCE: spec ms-vbal 5.6.9.8.4 Xor operator
' EXPECT: False
' EXPECT: True
' EXPECT: 6
Public Sub Main()
    Debug.Print CStr(True Xor True)
    Debug.Print CStr(True Xor False)
    Debug.Print CStr(12 Xor 10)
End Sub
