' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 reserved-name-member-dcl allows reserved words as member names
' XFAIL: wrong not implemented: reserved words such as Date as Type member names
' EXPECT: 4 abc
Private Type Rec
    Name As String
    Date As Long
End Type
Public Sub Main()
    Dim r As Rec
    r.Name = "abc"
    r.Date = 4
    Debug.Print CStr(r.Date) & " " & r.Name
End Sub
