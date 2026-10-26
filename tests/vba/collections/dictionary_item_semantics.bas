' INV: obj.dictionary, type.dictionary
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/item-property-dictionary-object
' EXPECT: True|True|1
' EXPECT: 5|2
' EXPECT: 7|2
' GRANT: com-create
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    ' Reading a missing key creates it, left empty.
    Debug.Print IsEmpty(d("x")) & "|" & d.Exists("x") & "|" & d.Count
    ' Assigning a missing key creates it; assigning an existing one replaces.
    d("y") = 5
    Debug.Print d("y") & "|" & d.Count
    d.Item("y") = 7
    Debug.Print d.Item("y") & "|" & d.Count
End Sub
