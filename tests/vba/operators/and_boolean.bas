' INV: op.and
' SOURCE: derived truth table
' EXPECT: True
' EXPECT: False
' EXPECT: False
Public Sub Main()
    Debug.Print CStr(True And True)
    Debug.Print CStr(True And False)
    Debug.Print CStr(False And False)
End Sub
