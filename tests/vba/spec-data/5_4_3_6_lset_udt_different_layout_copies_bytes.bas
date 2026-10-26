' INV: vbal.5.4.3.6, stmt.lset
' SOURCE: derived ms-vbal 5.4.3.6 LSet copies the bytes of a user-defined type over another; members are laid out at their natural alignment, little endian (the layout of a Declare user-defined type)
' EXPECT: 1 2 7
' EXPECT: 65 66 CD
Private Type A
    x As Long
    y As Long
End Type
Private Type B
    p As Integer
    q As Integer
    r As Long
End Type
Private Type S
    s As String * 4
End Type
Private Type T
    b1 As Byte
    b2 As Byte
    rest As String * 2
End Type
Public Sub Main()
    Dim x As A, y As B
    x.x = &H20001
    x.y = 7
    LSet y = x
    Debug.Print CStr(y.p) & " " & CStr(y.q) & " " & CStr(y.r)
    Dim u As S, v As T
    u.s = "ABCD"
    LSet v = u
    Debug.Print CStr(v.b1) & " " & CStr(v.b2) & " " & v.rest
End Sub
