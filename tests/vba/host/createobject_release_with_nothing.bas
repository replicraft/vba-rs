' INV: func.createobject, vbal.6.1.2.8.1.4
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/createobject-function
' METHOD: by hand
' GRANT: com-create
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    Debug.Print d Is Nothing
    Set d = Nothing
    Debug.Print d Is Nothing
End Sub
