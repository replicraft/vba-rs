' INV: vbal.5.4.3.9, stmt.set
' SOURCE: spec ms-vbal 5.4.3.9 Set into a Variant variable
' EXPECT: 6
'#MODULE C1 class
Public V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim v As Variant
    Dim c As New C1
    c.V = 6
    Set v = c
    Debug.Print CStr(v.V)
End Sub
