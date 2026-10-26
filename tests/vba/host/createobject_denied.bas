' INV: func.createobject, obj.err
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' ERROR: 70
Public Sub Main()
    Dim o As Object
    Set o = CreateObject("Scripting.Dictionary")
End Sub
