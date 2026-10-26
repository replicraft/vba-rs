' INV: vbal.5.6.9.5, op.lt
' SOURCE: spec ms-vbal 5.6.9.5 relational Boolean with Byte or Empty uses Integer
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Dim e As Variant
    Debug.Print CStr(True < CByte(255))
    Debug.Print CStr(CByte(255) > True)
    Debug.Print CStr(True < e)
    Debug.Print CStr(e > True)
    Debug.Print CStr(CInt(-1) = True)
End Sub
