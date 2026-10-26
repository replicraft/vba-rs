' INV: vbal.5.3.1.1
' SOURCE: spec ms-vbal 5.3.1.1 Friend procedure accessible within the project
' EXPECT: friend=7
'#MODULE Cls class
Friend Function Seven() As Long
    Seven = 7
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New Cls
    Debug.Print "friend=" & CStr(c.Seven())
End Sub
