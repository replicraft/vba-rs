' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 Set on the enclosing function sets its return value
' EXPECT: 3
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Private Function Make() As C1
    Dim t As New C1
    t.V = 3
    Set Make = t
End Function
Public Sub Main()
    Debug.Print CStr(Make().V)
End Sub
