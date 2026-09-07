//! Module de gestion des chaines de caractères en mémoire.

/// Module d'extraction de chaînes de caractère de la mémoire.
pub mod string_extraction {
    /// Extrait une chaîne de caractère (pointeur + taille de chaîne) de la mémoire utilisateur.
    ///
    /// # Arguments
    /// * `str_addr`: adresse du premier caractère de la chaine.
    /// * `str_len`: taille de la chaine à extraire.
    /// * `max_str_len`: taille max de la chaîne à extraire, si on extrait plus, erreur.
    ///
    /// # Return
    /// Si aucun problème, chaîne extraite de la mémoire,
    /// Sinon,
    ///     InvalidStringFormat si erreur de str::from_utf8
    ///     ServiceDenial 
    ///         si str_len > max_len
    ///         si current_addr not in user pages
    ///
    /// # Safety
    /// L'appelant doit être sûr que l'adresse en paramètre pointe bien vers la chaîne voulue.
    pub unsafe fn extract_str_with_len(
        str_addr: u64,
        str_len: usize,
        max_str_len: usize
    ) -> StringExtractionResult<&'static str> {
        if str_len > max_str_len {
            return Err(StringExtractionError::ServiceDenial)
        }

        let str_ptr = str_addr as *const u8;

        let byte_slice = core::slice::from_raw_parts(str_ptr, str_len);
        match str::from_utf8(byte_slice) {
            Ok(valid_str) => Ok(valid_str),
            Err(_) => Err(StringExtractionError::InvalidStringFormat)
        }
    }

    /// Wrapper rust sur les erreurs de chaîne
    pub type StringExtractionResult<T> = Result<T, StringExtractionError>;

    /// Type d'erreur liée à la manipulation de chaine.
    pub enum StringExtractionError {
        /// Tentative d'attaque par déni de service.
        ServiceDenial,

        /// Format de chaîne invalide,
        /// pour l'instant, ne supporte que le utf-8
        InvalidStringFormat
    }
}
