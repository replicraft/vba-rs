' INV: decl.dim-as-new
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dim-statement
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
