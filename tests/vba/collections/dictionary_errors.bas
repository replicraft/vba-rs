' INV: obj.dictionary, type.dictionary
' SOURCE: unverified the pages say only that an error occurs (duplicate key in Add, missing key in Remove and Key, CompareMode of a dictionary with data); the numbers 457, 32811 and 5 are assumptions
' EXPECT: 457
' EXPECT: 32811
' EXPECT: 32811
' EXPECT: 457
' EXPECT: 5
' GRANT: com-create
Public Sub Main()
    Dim d As Object
    Set d = CreateObject("Scripting.Dictionary")
    d.Add "a", 1
    d.Add "b", 2
    On Error Resume Next
    d.Add "a", 3
    Debug.Print Err.Number & ""
    Err.Clear
    d.Remove "zzz"
    Debug.Print Err.Number & ""
    Err.Clear
    d.Key("zzz") = "q"
    Debug.Print Err.Number & ""
    Err.Clear
    d.Key("a") = "b"
    Debug.Print Err.Number & ""
    Err.Clear
    d.CompareMode = 1
    Debug.Print Err.Number & ""
End Sub
