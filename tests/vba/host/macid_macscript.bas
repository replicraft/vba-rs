' INV: func.macid, func.macscript
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/macid-function
' ERROR: 5
' XFAIL: unsupported Macintosh-only functions
Public Sub Main()
    Debug.Print MacID("TEXT")
End Sub
