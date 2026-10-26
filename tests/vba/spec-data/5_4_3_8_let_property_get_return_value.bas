' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 assigning the enclosing Property Get sets its return value
' EXPECT: 9
'#MODULE C1 class
Public Property Get P() As Long
    P = 9
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    Debug.Print CStr(c.P)
End Sub
