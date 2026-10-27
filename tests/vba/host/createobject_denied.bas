' INV: func.createobject, obj.err, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' ERROR: 70
Public Sub Main()
    Dim o As Object
    Set o = CreateObject("Scripting.Dictionary")
End Sub
