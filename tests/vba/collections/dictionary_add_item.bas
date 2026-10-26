' INV: type.dictionary, obj.dictionary
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/dictionary-object
' EXPECT:  1 
' GRANT: com-create
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", 1
    Debug.Print d("a")
End Sub
