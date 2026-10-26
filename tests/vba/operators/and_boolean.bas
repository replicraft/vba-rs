' INV: op.and
' SOURCE: spec ms-vbal 5.6.9.8.2 And operator
' EXPECT: True
' EXPECT: False
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(True And True)
    Debug.Print CStr(True And False)
    Debug.Print CStr(False And False)
End Sub
