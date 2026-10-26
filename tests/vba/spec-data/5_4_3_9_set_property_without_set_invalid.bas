' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 property without accessible Property Set is invalid target
' ERROR: load
'#MODULE C1 class
Public V As Long
'#MODULE C2 class
Public Property Get Child() As C1
    Set Child = New C1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim p As New C2
    Set p.Child = New C1
End Sub
