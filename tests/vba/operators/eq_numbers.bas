' INV: op.eq
' SOURCE: spec ms-vbal 5.6.9.5.1 equality operator
' EXPECT: True
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(1 = 1)
    Debug.Print CStr(1 = 2)
    Debug.Print CStr(1 = 1.0)
End Sub
