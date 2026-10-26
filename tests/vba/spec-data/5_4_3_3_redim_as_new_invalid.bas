' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 As clause must be as-type, not as-auto-object
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: ReDim ... As New is accepted
' ERROR: load
'#MODULE C1 class
Public x As Integer
'#MODULE Module1 standard
Public Sub Main()
    Dim a() As C1
    ReDim a(2) As New C1
End Sub
