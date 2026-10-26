' INV: vbal.3.3.5, lex.case-insensitive
' SOURCE: spec ms-vbal 3.3.5.2 name values, two names differing only in case are the same name, so redeclaring one in the same scope is a conflict
' ERROR: load
Public Sub Main()
    Dim abc As Long
    Dim ABC As Long
End Sub
