' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 l-expression must be variable, property or unbound member
' ERROR: load
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Const k As Long = 1
    Dim c As New C1
    Set k = c
End Sub
