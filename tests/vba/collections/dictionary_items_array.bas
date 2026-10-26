' INV: obj.dictionary, type.dictionary
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/items-method
' EXPECT: 0|2|Athens Belgrade Cairo
' EXPECT: 0|2|a b c
' EXPECT: -1
' GRANT: com-create
Public Sub Main()
    Dim a As Variant, d As Object, i As Long, s As String
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", "Athens"
    d.Add "b", "Belgrade"
    d.Add "c", "Cairo"
    a = d.Items
    For i = 0 To d.Count - 1
        s = s & IIf(i = 0, "", " ") & a(i)
    Next
    Debug.Print LBound(a) & "|" & UBound(a) & "|" & s
    a = d.Keys
    s = ""
    For i = LBound(a) To UBound(a)
        s = s & IIf(i = 0, "", " ") & a(i)
    Next
    Debug.Print LBound(a) & "|" & UBound(a) & "|" & s
    d.RemoveAll
    a = d.Keys
    Debug.Print CStr(UBound(a))
End Sub
