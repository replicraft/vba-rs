' INV: vbal.5.6.9.5, op.lt, op.gt
' SOURCE: spec ms-vbal 5.6.9.5 relational Boolean effective type, True less than False
' EXPECT: True
' EXPECT: False
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(True < False)
    Debug.Print CStr(True > False)
    Debug.Print CStr(False > True)
    Debug.Print CStr(True = "True")
    Debug.Print CStr("False" = False)
    Debug.Print CStr(True <> False)
End Sub
