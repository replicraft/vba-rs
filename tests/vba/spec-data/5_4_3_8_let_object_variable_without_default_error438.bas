' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 Let on class-typed variable without default property is error 438
' XFAIL: wrong pending
' ERROR: 438
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim c As New C1
    c = 42
End Sub
