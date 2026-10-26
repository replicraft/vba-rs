' INV: obj.err
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/err-object
' EXPECT: True
Public Sub Main()
    On Error Resume Next
    Err.Raise vbObjectError + 513, "src", "boom"
    Debug.Print Err.Number = vbObjectError + 513
End Sub
