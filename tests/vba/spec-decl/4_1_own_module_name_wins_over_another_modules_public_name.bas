' INV: vbal.4.1
' SOURCE: derived 4.1 a name declared in the module that uses it is found before a Public name of another module
' METHOD: by hand
' EXPECT: Module1
' EXPECT: Helper
'#MODULE Helper standard
Public Function Where() As String
    Where = "Helper"
End Function
'#MODULE Module1 standard
Private Function Where() As String
    Where = "Module1"
End Function
Public Sub Main()
    Debug.Print Where()
    Debug.Print Helper.Where()
End Sub
