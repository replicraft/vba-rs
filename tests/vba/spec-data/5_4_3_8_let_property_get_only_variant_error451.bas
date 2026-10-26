' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 no Property Let but Property Get of Variant type gives error 451
' XFAIL: wrong VBA behaviour kept where MS-VBAL text differs: misuse of Let on objects and read-only properties is rejected at analysis, not at run time
' ERROR: 451
'#MODULE C1 class
Public Property Get P() As Variant
    P = 1
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    c.P = 2
End Sub
