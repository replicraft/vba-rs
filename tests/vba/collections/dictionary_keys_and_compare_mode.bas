' INV: obj.dictionary, type.dictionary
' SOURCE: unverified binary versus text key comparison follows the CompareMode page; numeric keys of different subtypes comparing equal, and a number never equaling a string, are assumptions
' EXPECT: False|True
' EXPECT: True|True
' EXPECT: True|False
' EXPECT: 1|b|x
' GRANT: com-create
Public Sub Main()
    Dim d As Object, e As Object, k As Variant
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "Key", 1
    Debug.Print d.Exists("key") & "|" & d.Exists("Key")
    Set e = CreateObject("Scripting.Dictionary")
    e.CompareMode = 1
    e.Add "Key", 1
    Debug.Print e.Exists("key") & "|" & e.Exists("KEY")
    Set d = CreateObject("Scripting.Dictionary")
    d.Add 1, "one"
    Debug.Print d.Exists(1&) & "|" & d.Exists("1")
    ' A key can be renamed in place; the item and the position stay.
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", "x"
    d.Key("a") = "b"
    For Each k In d
        Debug.Print d.Count & "|" & k & "|" & d(k)
    Next k
End Sub
