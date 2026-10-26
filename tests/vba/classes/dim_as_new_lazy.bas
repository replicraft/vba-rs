' INV: decl.dim-as-new
' SOURCE: spec ms-vbal 2.5.1 automatic object instantiation, 5.3.1.10 Class_Initialize lifecycle handler
' EXPECT: declared
' EXPECT: init
' EXPECT: used
'#MODULE Res class
Public X As Long
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim r As New Res
    Debug.Print "declared"
    r.X = 1
    Debug.Print "used"
End Sub
