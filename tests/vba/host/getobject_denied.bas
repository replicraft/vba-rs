' INV: func.getobject, vbal.6.1.2.8.1.9
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/getobject-function
' ERROR: 70
Public Sub Main()
    Dim o As Object
    Set o = GetObject("/some/file.xls")
End Sub
