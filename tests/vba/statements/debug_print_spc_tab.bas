' INV: obj.debug, func.spc, func.tab
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/debug-object
' EXPECT: a   b
' EXPECT: a   b
Public Sub Main()
    Debug.Print "a"; Spc(3); "b"
    Debug.Print "a"; Tab(5); "b"
End Sub
