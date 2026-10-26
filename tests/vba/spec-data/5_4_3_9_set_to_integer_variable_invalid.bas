' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 Set-coercion to a non-object declared type is invalid
' ERROR: load
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim n As Long
    Dim c As New C1
    Set n = c
End Sub
