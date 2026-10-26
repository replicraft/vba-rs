' INV: vbal.5.2.3.3, vbal.5.2.3.1.4
' SOURCE: spec ms-vbal 5.2.3.3 UDT member with fixed-length string type
' EXPECT: 4
Private Type Rec
    code As String * 4
End Type
Public Sub Main()
    Dim r As Rec
    Debug.Print CStr(Len(r.code))
End Sub
